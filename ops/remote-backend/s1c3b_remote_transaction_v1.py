#!/usr/bin/env python3
"""Root-only executor for the frozen S1C-3B production-load transaction."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

try:
    import s1c3_remote_transaction_v7 as legacy
except ModuleNotFoundError:
    import importlib.util

    _legacy_path = Path(__file__).with_name("s1c3_remote_transaction_v7.py")
    _legacy_spec = importlib.util.spec_from_file_location("s1c3_remote_transaction_v7", _legacy_path)
    if _legacy_spec is None or _legacy_spec.loader is None:
        raise
    legacy = importlib.util.module_from_spec(_legacy_spec)
    _legacy_spec.loader.exec_module(legacy)


PAPER_COMMIT = "36ffc0cbf56b72b2c07ff97c83bb5ac271ed5189"
PAPER_TREE = "1f8a9c7fc0cdd572a3adf7ba4a7ad294b47031ad"
PAPER_MANIFEST_ROOT = "3b98c93828d5260397365373e742542869b2419f050c3d08a21947cbb207e5b6"
PAPER_VERIFICATION_SHA256 = "6c5f87233fadbfac03671dab7f5a652d1597618434efa4fd16bfddb874ec8e26"
PAPER_CRITIQUE_SHA256 = "3898ce907d4b6a68c21fb4fd435dfa5f9ba5af3179787a949bb745c279a8e2ef"

CANDIDATE_COMMIT = legacy.CANDIDATE_COMMIT
CANDIDATE_TREE = legacy.CANDIDATE_TREE
CARGO_LOCK_SHA256 = legacy.CARGO_LOCK_SHA256
CANDIDATE_CONFIG_SHA256 = legacy.CANDIDATE_CONFIG_SHA256
PRODUCTION_PROJECTION_PATH = legacy.PRODUCTION_PROJECTION_PATH
PRODUCTION_PROJECTION_SCHEMA = legacy.PRODUCTION_PROJECTION_SCHEMA
PRODUCTION_PROJECTION_SHA256 = legacy.PRODUCTION_PROJECTION_SHA256
BASELINE_COMMIT = legacy.BASELINE_COMMIT
BASELINE_TREE = legacy.BASELINE_TREE
BASELINE_BINARY_SHA256 = legacy.BASELINE_BINARY_SHA256
BASELINE_CONFIG_SHA256 = legacy.BASELINE_CONFIG_SHA256

CURRENT_RECEIPT = Path(
    "/var/lib/nando-wave/deployments/20260812T061317Z-b409fa339ee5/deployment-receipt.json"
)
CURRENT_RECEIPT_ROOT = "d02a3d7ad31e73aa005467806357b172cacac73b6a4d96122518ca642cf15245"
CURRENT_RECEIPT_FILE_SHA256 = "9f63d3e5addd186d59061ee4cc6bbf894c510cde0d813054fdfc96a033ad1db6"
CURRENT_RECEIPT_COMMIT = "b409fa339ee5f2d07ffbc18917f474188000a743"
CURRENT_RECEIPT_TREE = "a9b948836c054e52a0421c6116c56144168e26cc"

MEASUREMENT_CPU = 4
MEASUREMENT_SIBLING = 5
MONITOR_INTERVAL_SECONDS = 0.5
MONITOR_MAX_GAP_SECONDS = 2.0
FLOOR_RECORDS = 256
ROUND_COUNT = 3

HOT_TEST = legacy.HOT_TEST
IDLE_TEST = legacy.IDLE_TEST
IDLE_METRIC_FIELDS = ("elapsed_ticks", "ticks_per_second", "percent_of_one_core")
SINGLE_SYNC_TEST = legacy.SINGLE_SYNC_TEST
THREE_SYNC_TEST = legacy.THREE_SYNC_TEST
HOT_RE = legacy.HOT_RE
IDLE_RE = legacy.IDLE_RE
SYNC_RE = legacy.SYNC_RE
STAGE_SYNC_RE = legacy.STAGE_SYNC_RE
AFFINITY_RE = re.compile(r"S1C3B_AFFINITY pid=(\d+) cpus=([0-9,]+) executable_sha256=([0-9a-f]{64})")

MEASUREMENT_LABELS = tuple(
    label
    for round_index in range(1, ROUND_COUNT + 1)
    for label in (
        f"floor-before-{round_index}",
        f"hot-{round_index}",
        f"single-sync-{round_index}",
        f"three-sync-{round_index}",
        f"floor-after-{round_index}",
    )
) + (
    "idle",
    "rss-capture_off",
    "rss-capture_on",
    "parity-baseline",
    "parity-candidate",
)

OWNERSHIP_SCHEMA = "nando.s1c3b-oracle-ownership-receipt.v1"
OWNERSHIP_ROW_SCHEMA = "nando.s1c3b-oracle-ownership-row.v1"
legacy.OWNERSHIP_SCHEMA = OWNERSHIP_SCHEMA
legacy.OWNERSHIP_ROW_SCHEMA = OWNERSHIP_ROW_SCHEMA
PROCESS_SNAPSHOT_SCHEMA = legacy.PROCESS_SNAPSHOT_SCHEMA
PROCESS_DETECTOR_SCHEMA = legacy.PROCESS_DETECTOR_SCHEMA
MONITOR_SCHEMA = "nando.s1c3b-production-load-monitor.v1"
RESOURCE_SCHEMA = "nando.s1c3b-resource-receipt.v1"
PARITY_SCHEMA = "nando.s1c3b-parity-receipt.v1"
PREPARATION_SCHEMA = "nando.s1c3b-transaction-preparation.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3b-predeployment-verification.v1"
RECEIPT_SCHEMA = "nando.s1c3b-transaction-receipt.v1"
STATE_SCHEMA = "nando.s1c3b-state.v1"
PENDING_SCHEMA = "nando.s1c3b-pending-receipt.v1"
FINAL_VERIFICATION_SCHEMA = "nando.s1c3b-verification.v1"

PRODUCTION_BINARY = legacy.PRODUCTION_BINARY
PRODUCTION_CONFIG = legacy.PRODUCTION_CONFIG
UNIT_FILE = legacy.UNIT_FILE
PHASE_CONFIG = legacy.PHASE_CONFIG
AUTHORITY_CONFIG = legacy.AUTHORITY_CONFIG
JOURNAL = legacy.JOURNAL
TRANSITION_UNIT = legacy.TRANSITION_UNIT
UNTOUCHED_UNITS = legacy.UNTOUCHED_UNITS

GateFailure = legacy.GateFailure
utc_now = legacy.utc_now
sha256_bytes = legacy.sha256_bytes
sha256_file = legacy.sha256_file
canonical_bytes = legacy.canonical_bytes
add_root = legacy.add_root
atomic_write = legacy.atomic_write
write_json = legacy.write_json
read_json = legacy.read_json
run = legacy.run
systemctl = legacy.systemctl
fsync_path = legacy.fsync_path
fsync_directory = legacy.fsync_directory
service_snapshot = legacy.service_snapshot
require_active = legacy.require_active
health_snapshot = legacy.health_snapshot
economics_snapshot = legacy.economics_snapshot
route_probe = legacy.route_probe
journal_snapshot = legacy.journal_snapshot
process_rss = legacy.process_rss
process_environment = legacy.process_environment
parse_env_file = legacy.parse_env_file
install_pair = legacy.install_pair
wait_for_service = legacy.wait_for_service
executable_identity = legacy.executable_identity
build_process_snapshot = legacy.build_process_snapshot
prebuild_test_harness = legacy.prebuild_test_harness
prebuild_oracle = legacy.prebuild_oracle
build_ownership_receipt = legacy.build_ownership_receipt
production_projection_sha256 = legacy.production_projection_sha256


AFFINITY_WRAPPER_SOURCE = b'''#!/usr/bin/env python3
import hashlib, os, sys
target = sys.argv[1]
with open(target, "rb") as handle:
    root = hashlib.sha256(handle.read()).hexdigest()
print("S1C3B_AFFINITY pid=%d cpus=%s executable_sha256=%s" % (
    os.getpid(), ",".join(str(cpu) for cpu in sorted(os.sched_getaffinity(0))), root
), flush=True)
os.execv(target, [target, *sys.argv[2:]])
'''

FLOOR_PROBE_SOURCE = b'''#!/usr/bin/env python3
import json, os, sys, time
root = sys.argv[1]
samples = []
payload = b"S1C3B-FILESYSTEM-FLOOR".ljust(4096, b"\\0")
for index in range(256):
    path = os.path.join(root, "%04d.bin" % index)
    started = time.monotonic_ns()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(fd, payload)
        os.fdatasync(fd)
    finally:
        os.close(fd)
    os.unlink(path)
    samples.append(time.monotonic_ns() - started)
print(json.dumps({"samples_ns": samples}, sort_keys=True))
'''


def pressure_snapshot(kind: str) -> dict[str, dict[str, float | int]]:
    result: dict[str, dict[str, float | int]] = {}
    for line in Path(f"/proc/pressure/{kind}").read_text(encoding="ascii").splitlines():
        parts = line.split()
        row: dict[str, float | int] = {}
        for item in parts[1:]:
            key, raw = item.split("=", 1)
            row[key] = int(raw) if key == "total" else float(raw)
        result[parts[0]] = row
    if "some" not in result or (kind == "io" and "full" not in result):
        raise GateFailure(f"pressure_shape_invalid:{kind}")
    return result


def memory_available_bytes() -> int:
    rows = dict(
        line.split(":", 1) for line in Path("/proc/meminfo").read_text(encoding="ascii").splitlines()
    )
    value = rows.get("MemAvailable", "").strip().split()
    if len(value) != 2 or value[1] != "kB":
        raise GateFailure("mem_available_shape_invalid")
    return int(value[0]) * 1024


def block_device_counters() -> list[dict[str, Any]]:
    rows = []
    for line in Path("/proc/diskstats").read_text(encoding="ascii").splitlines():
        fields = line.split()
        if len(fields) < 14:
            raise GateFailure("diskstats_shape_invalid")
        rows.append({
            "major": int(fields[0]),
            "minor": int(fields[1]),
            "name": fields[2],
            "reads_completed": int(fields[3]),
            "sectors_read": int(fields[5]),
            "writes_completed": int(fields[7]),
            "sectors_written": int(fields[9]),
            "io_ms": int(fields[12]),
        })
    return rows


def cpu_counters() -> dict[str, dict[str, int]]:
    return legacy.cpu_counters((MEASUREMENT_CPU, MEASUREMENT_SIBLING))


def parse_parent_pid(stat_raw: str) -> int:
    closing = stat_raw.rfind(")")
    tail = stat_raw[closing + 2 :].split()
    if closing < 0 or len(tail) < 2:
        raise ValueError("stat_parent_shape_invalid")
    return int(tail[1])


def transaction_owned_builds(snapshot: dict[str, Any], executor_pid: int) -> list[dict[str, Any]]:
    parents: dict[int, int] = {}
    rows: dict[int, dict[str, Any]] = {}
    for row in snapshot["rows"]:
        opening = row.get("opening_stat", {})
        if opening.get("status") != "VALUE":
            continue
        try:
            parents[row["pid"]] = parse_parent_pid(opening["raw"])
            rows[row["pid"]] = row
        except (KeyError, TypeError, ValueError):
            continue
    owned = []
    for pid, row in rows.items():
        cursor = pid
        seen = set()
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


def host_observation(label: str, executor_pid: int) -> dict[str, Any]:
    processes = build_process_snapshot()
    counters = cpu_counters()
    return {
        "label": label,
        "observed_at": utc_now(),
        "monotonic_ns": time.monotonic_ns(),
        "cpu_counters": counters,
        "cpu_snapshot_root_sha256": sha256_bytes(canonical_bytes(counters)),
        "cpu_pressure": pressure_snapshot("cpu"),
        "io_pressure": pressure_snapshot("io"),
        "loadavg": Path("/proc/loadavg").read_text(encoding="ascii").strip(),
        "memory_available_bytes": memory_available_bytes(),
        "block_device_counters": block_device_counters(),
        "process_snapshot": processes,
        "process_snapshot_root_sha256": processes["process_snapshot_root_sha256"],
        "foreign_build_processes": processes["forbidden_process_matches"],
        "unresolved_processes": processes["unresolved_rows"],
        "transaction_owned_build_processes": transaction_owned_builds(processes, executor_pid),
        "observer_affinity": sorted(os.sched_getaffinity(0)),
    }


class MeasurementMonitor:
    def __init__(self, transaction_id: str, executable_root: str) -> None:
        self.transaction_id = transaction_id
        self.executable_root = executable_root
        self.executor_pid = os.getpid()
        self.samples: list[dict[str, Any]] = []
        self.boundaries: list[dict[str, Any]] = []
        self.commands: list[dict[str, Any]] = []
        self.errors: list[str] = []
        self.current_label = "before-first-metric"
        self.stop_event = threading.Event()
        self.lock = threading.Lock()
        self.sample_lock = threading.Lock()
        self.thread = threading.Thread(target=self._loop, name="s1c3b-monitor", daemon=True)

    def _sample(self, kind: str) -> int | None:
        with self.sample_lock:
            try:
                with self.lock:
                    label = self.current_label
                row = host_observation(label, self.executor_pid)
                row["kind"] = kind
                with self.lock:
                    self.samples.append(row)
                    return len(self.samples) - 1
            except Exception as error:
                with self.lock:
                    self.errors.append(f"{type(error).__name__}:{error}")
                return None

    def _loop(self) -> None:
        while not self.stop_event.wait(MONITOR_INTERVAL_SECONDS):
            self._sample("periodic")

    def start(self) -> None:
        self._sample("monitor-start")
        self.thread.start()

    def boundary(self, label: str, phase: str) -> None:
        with self.lock:
            self.current_label = label
        sample_index = self._sample(f"boundary-{phase}")
        with self.lock:
            sample = self.samples[sample_index] if sample_index is not None else {}
            self.boundaries.append({
                "label": label,
                "phase": phase,
                "observed_at": sample.get("observed_at"),
                "monotonic_ns": sample.get("monotonic_ns"),
                "sample_index": sample_index,
            })

    def command(self, row: dict[str, Any]) -> None:
        with self.lock:
            self.commands.append(row)

    def finish(self, measurement_started_at: str, measurement_started_ns: int) -> dict[str, Any]:
        self.stop_event.set()
        self.thread.join(timeout=5)
        if self.thread.is_alive():
            self.errors.append("monitor_thread_did_not_stop")
        self._sample("monitor-stop")
        ordered = sorted(self.samples, key=lambda row: row["monotonic_ns"])
        gaps = [
            (right["monotonic_ns"] - left["monotonic_ns"]) / 1_000_000_000
            for left, right in zip(ordered, ordered[1:])
        ]
        owned = [
            {"sample_index": index, "process": process}
            for index, row in enumerate(ordered)
            for process in row["transaction_owned_build_processes"]
        ]
        receipt = {
            "schema": MONITOR_SCHEMA,
            "transaction_id": self.transaction_id,
            "executor_pid": self.executor_pid,
            "measurement_cpu": MEASUREMENT_CPU,
            "measurement_sibling": MEASUREMENT_SIBLING,
            "measurement_started_at": measurement_started_at,
            "measurement_started_monotonic_ns": measurement_started_ns,
            "measurement_finished_at": utc_now(),
            "measurement_finished_monotonic_ns": time.monotonic_ns(),
            "monitor_interval_seconds": MONITOR_INTERVAL_SECONDS,
            "maximum_sample_gap_seconds": MONITOR_MAX_GAP_SECONDS,
            "observed_max_sample_gap_seconds": max(gaps, default=0.0),
            "metric_labels": list(MEASUREMENT_LABELS),
            "boundaries": self.boundaries,
            "commands": self.commands,
            "samples": ordered,
            "transaction_owned_build_processes": owned,
            "monitor_errors": self.errors,
            "instrument_pass": not self.errors
            and not owned
            and max(gaps, default=0.0) <= MONITOR_MAX_GAP_SECONDS,
            "executable_set_root_sha256": self.executable_root,
        }
        return add_root(receipt, "monitor_root_sha256")


def write_measurement_scripts(work: Path) -> tuple[Path, Path]:
    wrapper = work / "s1c3b-affinity-exec.py"
    probe = work / "s1c3b-filesystem-floor.py"
    atomic_write(wrapper, AFFINITY_WRAPPER_SOURCE, 0o500)
    atomic_write(probe, FLOOR_PROBE_SOURCE, 0o500)
    shutil.chown(wrapper, user="e", group="e")
    shutil.chown(probe, user="e", group="e")
    return wrapper, probe


def run_measured(
    executable: Path,
    arguments: list[str],
    label: str,
    source: Path | None,
    evidence: Path,
    timeout: int,
    monitor: MeasurementMonitor,
    wrapper: Path,
    env: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[bytes] | None, str, dict[str, Any]]:
    monitor.boundary(label, "before")
    error = ""
    completed: subprocess.CompletedProcess[bytes] | None = None
    output = b""
    try:
        completed = run(
            ["taskset", "-c", str(MEASUREMENT_CPU), "/usr/bin/python3", str(wrapper), str(executable), *arguments],
            cwd=source,
            env=env,
            as_user=True,
            timeout=timeout,
            check=False,
        )
        output = completed.stdout
    except Exception as caught:
        error = f"{type(caught).__name__}:{caught}"
    finally:
        monitor.boundary(label, "after")
    legacy.atomic_write(evidence / f"{label}.log", output, 0o400)
    text = output.decode("utf-8", "replace")
    affinity = AFFINITY_RE.search(text)
    command = {
        "label": label,
        "executable": str(executable),
        "executable_sha256": sha256_file(executable),
        "requested_affinity": [MEASUREMENT_CPU],
        "observed_affinity": [int(cpu) for cpu in affinity.group(2).split(",")] if affinity else None,
        "wrapper_reported_executable_sha256": affinity.group(3) if affinity else None,
        "returncode": completed.returncode if completed is not None else None,
        "error": error or None,
    }
    monitor.command(command)
    return completed, text, command


def test_metric(
    executable: Path,
    source: Path,
    test: str,
    label: str,
    expression: re.Pattern[str],
    fields: tuple[str, ...],
    evidence: Path,
    timeout: int,
    monitor: MeasurementMonitor,
    wrapper: Path,
) -> dict[str, Any]:
    completed, output, command = run_measured(
        executable,
        [test, "--ignored", "--exact", "--nocapture", "--test-threads=1"],
        label,
        source,
        evidence,
        timeout,
        monitor,
        wrapper,
        {"RUST_TEST_THREADS": "1"},
    )
    match = expression.search(output)
    if match is None:
        metrics = None
    elif label == "idle":
        values: list[Any] = [int(match.group(1)), int(match.group(2)), float(match.group(3))]
        metrics = dict(zip(fields, values, strict=True))
    else:
        metrics = dict(zip(fields, map(int, match.groups()), strict=True))
    return {
        "label": label,
        "test": test,
        "returncode": completed.returncode if completed is not None else None,
        "test_assertion_pass": completed is not None and completed.returncode == 0,
        "metric_present": match is not None,
        "metrics": metrics,
        "output_sha256": sha256_bytes(output.encode("utf-8")),
        "command": command,
    }


def percentile(samples: list[int], percent: int) -> int:
    ordered = sorted(samples)
    index = max(0, (len(ordered) * percent + 99) // 100 - 1)
    return ordered[index]


def filesystem_identity(path: Path) -> dict[str, Any]:
    stat = os.stat(path)
    statvfs = os.statvfs(path)
    completed = run(["findmnt", "-T", str(path), "-n", "-o", "SOURCE,FSTYPE,OPTIONS"], check=False)
    return {
        "device": stat.st_dev,
        "filesystem_id": statvfs.f_fsid,
        "block_size": statvfs.f_bsize,
        "findmnt": completed.stdout.decode("utf-8", "replace").strip(),
        "findmnt_returncode": completed.returncode,
    }


def floor_probe(
    round_index: int,
    position: str,
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
    wrapper: Path,
    probe: Path,
    filesystem: dict[str, Any],
) -> dict[str, Any]:
    label = f"floor-{position}-{round_index}"
    directory = work / label
    directory.mkdir(mode=0o700)
    shutil.chown(directory, user="e", group="e")
    completed, output, command = run_measured(
        Path("/usr/bin/python3"),
        [str(probe), str(directory)],
        label,
        None,
        evidence,
        300,
        monitor,
        wrapper,
    )
    samples: list[int] = []
    error = None
    try:
        payload = json.loads(output.splitlines()[-1])
        samples = payload["samples_ns"]
        if len(samples) != FLOOR_RECORDS or not all(isinstance(value, int) and value >= 0 for value in samples):
            raise ValueError("floor_sample_shape_invalid")
    except Exception as caught:
        error = f"{type(caught).__name__}:{caught}"
    return {
        "label": label,
        "round": round_index,
        "position": position,
        "records": len(samples),
        "samples_ns": samples,
        "samples_root_sha256": sha256_bytes(canonical_bytes(samples)),
        "p50_ns": percentile(samples, 50) if samples else None,
        "p99_ns": percentile(samples, 99) if samples else None,
        "hard_max_ns": max(samples) if samples else None,
        "filesystem": filesystem,
        "diagnostic_only": True,
        "returncode": completed.returncode if completed is not None else None,
        "error": error,
        "command": command,
    }


def measure_rss_mode(
    candidate_binary: Path,
    config: dict[str, str],
    capture: bool,
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
    wrapper: Path,
    port: int,
) -> dict[str, Any]:
    label = "capture_on" if capture else "capture_off"
    metric_label = f"rss-{label}"
    state = work / f"rss-{label}"
    environment = legacy.isolated_environment(config, state, port, capture)
    log_path = evidence / f"{metric_label}.log"
    monitor.boundary(metric_label, "before")
    error = None
    rss_samples: list[int] = []
    output = b""
    account = legacy.pwd.getpwnam("e")

    def demote() -> None:
        os.initgroups(account.pw_name, account.pw_gid)
        os.setgid(account.pw_gid)
        os.setuid(account.pw_uid)

    command = [
        "taskset", "-c", str(MEASUREMENT_CPU), "/usr/bin/python3", str(wrapper), str(candidate_binary)
    ]
    with log_path.open("wb") as log:
        process = subprocess.Popen(
            command,
            env=environment,
            stdout=log,
            stderr=subprocess.STDOUT,
            preexec_fn=demote,
        )
        try:
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise GateFailure(f"rss_process_exited:{label}:{process.returncode}")
                try:
                    legacy.http_json(f"http://127.0.0.1:{port}/health")
                    break
                except Exception:
                    time.sleep(0.25)
            else:
                raise GateFailure(f"rss_health_timeout:{label}")
            for _ in range(20):
                rss_samples.append(process_rss(process.pid))
                time.sleep(0.1)
        except Exception as caught:
            error = f"{type(caught).__name__}:{caught}"
        finally:
            if process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    monitor.boundary(metric_label, "after")
    os.chmod(log_path, 0o400)
    output = log_path.read_bytes()
    affinity = AFFINITY_RE.search(output.decode("utf-8", "replace"))
    command_receipt = {
        "label": metric_label,
        "executable": str(candidate_binary),
        "executable_sha256": sha256_file(candidate_binary),
        "requested_affinity": [MEASUREMENT_CPU],
        "observed_affinity": [int(cpu) for cpu in affinity.group(2).split(",")] if affinity else None,
        "wrapper_reported_executable_sha256": affinity.group(3) if affinity else None,
        "returncode": process.returncode,
        "error": error,
    }
    monitor.command(command_receipt)
    return {
        "label": label,
        "rss_bytes": max(rss_samples) if rss_samples else None,
        "sample_count": len(rss_samples),
        "error": error,
        "command": command_receipt,
    }


def run_parity(
    oracles: dict[str, Path],
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
    wrapper: Path,
) -> dict[str, Any]:
    fixture = work / "parity-fixture"
    fixture.mkdir()
    shutil.copy2(legacy.RESPONSE_REGISTRY, fixture / "response-registry.json")
    shutil.copy2(legacy.ADMISSION, fixture / "admission.json")
    rows = []
    outputs: dict[str, bytes] = {}
    for label in ("baseline", "candidate"):
        completed, output, command = run_measured(
            oracles[label],
            [str(fixture / "response-registry.json"), str(fixture / "admission.json")],
            f"parity-{label}",
            None,
            evidence,
            1200,
            monitor,
            wrapper,
        )
        outputs[label] = output.encode("utf-8")
        rows.append({
            "label": label,
            "returncode": completed.returncode if completed is not None else None,
            "output_sha256": sha256_bytes(outputs[label]),
            "row_count": len(outputs[label].splitlines()) - 1,
            "command": command,
        })
    comparable = all(row["returncode"] == 0 for row in rows)
    identical = comparable and outputs["baseline"].splitlines()[1:] == outputs["candidate"].splitlines()[1:]
    row_count = rows[1]["row_count"]
    return add_root({
        "schema": PARITY_SCHEMA,
        "rows": rows,
        "byte_identical": identical,
        "row_count": row_count,
        "baseline_output_sha256": sha256_bytes(b"\n".join(outputs["baseline"].splitlines()[1:])),
        "candidate_output_sha256": sha256_bytes(b"\n".join(outputs["candidate"].splitlines()[1:])),
    }, "parity_root_sha256")


def evaluate_measurement(
    candidate_binary: Path,
    source: Path,
    harnesses: dict[str, Path],
    config: dict[str, str],
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
    wrapper: Path,
    probe: Path,
    filesystem: dict[str, Any],
    executable_root: str,
    ownership_root: str,
    oracles: dict[str, Path],
) -> tuple[dict[str, Any], dict[str, Any]]:
    floors = []
    hot = []
    single = []
    three = []
    for round_index in range(1, ROUND_COUNT + 1):
        floors.append(floor_probe(round_index, "before", work, evidence, monitor, wrapper, probe, filesystem))
        hot.append(test_metric(
            harnesses["response-actor"], source, HOT_TEST, f"hot-{round_index}", HOT_RE,
            ("p99_ns", "no_goal_p99_ns", "hard_max_ns", "samples"), evidence, 600, monitor, wrapper,
        ))
        single.append(test_metric(
            harnesses["transition-serving"], source, SINGLE_SYNC_TEST, f"single-sync-{round_index}", SYNC_RE,
            ("p99_ns", "hard_max_ns", "samples", "segments"), evidence, 900, monitor, wrapper,
        ))
        three.append(test_metric(
            harnesses["transition-serving"], source, THREE_SYNC_TEST, f"three-sync-{round_index}", STAGE_SYNC_RE,
            ("precommit_p99_ns", "precommit_hard_max_ns", "settlement_p99_ns", "settlement_hard_max_ns", "episode_p99_ns", "episode_hard_max_ns", "samples"),
            evidence, 900, monitor, wrapper,
        ))
        floors.append(floor_probe(round_index, "after", work, evidence, monitor, wrapper, probe, filesystem))

    idle = test_metric(
        harnesses["response-actor"], source, IDLE_TEST, "idle", IDLE_RE,
        IDLE_METRIC_FIELDS, evidence, 180, monitor, wrapper,
    )
    rss_rows = [
        measure_rss_mode(candidate_binary, config, False, work, evidence, monitor, wrapper, 19871),
        measure_rss_mode(candidate_binary, config, True, work, evidence, monitor, wrapper, 19872),
    ]
    parity = run_parity(oracles, work, evidence, monitor, wrapper)

    resource_failures: list[str] = []
    instrument_failures: list[str] = []
    for row in floors:
        if row["records"] != FLOOR_RECORDS or row["returncode"] != 0 or row["error"]:
            instrument_failures.append(f'{row["label"]}:floor_incomplete')
    for rows, kind in ((hot, "hot"), (single, "single"), (three, "three")):
        for row in rows:
            label = row["label"]
            metric = row["metrics"]
            if not row["test_assertion_pass"]:
                instrument_failures.append(f"{label}:test_assertion_failed")
            if metric is None:
                instrument_failures.append(f"{label}:metric_missing")
                continue
            if row["command"]["observed_affinity"] != [MEASUREMENT_CPU]:
                instrument_failures.append(f"{label}:affinity_invalid")
            if kind == "hot":
                if metric["samples"] != 4096:
                    instrument_failures.append(f"{label}:denominator")
                if metric["p99_ns"] > 1_000_000:
                    resource_failures.append(f"{label}:matched_p99")
                if metric["no_goal_p99_ns"] > 250_000:
                    resource_failures.append(f"{label}:no_goal_p99")
                if metric["hard_max_ns"] > 2_000_000:
                    resource_failures.append(f"{label}:hard_max")
            elif kind == "single":
                if metric["samples"] != 1024:
                    instrument_failures.append(f"{label}:denominator")
                if metric["p99_ns"] > 5_000_000:
                    resource_failures.append(f"{label}:p99")
                if metric["hard_max_ns"] > 20_000_000:
                    resource_failures.append(f"{label}:hard_max")
            else:
                if metric["samples"] != 256:
                    instrument_failures.append(f"{label}:denominator")
                for field in ("precommit_p99_ns", "settlement_p99_ns"):
                    if metric[field] > 5_000_000:
                        resource_failures.append(f"{label}:{field}")
                for field in ("precommit_hard_max_ns", "settlement_hard_max_ns", "episode_hard_max_ns"):
                    if metric[field] > 20_000_000:
                        resource_failures.append(f"{label}:{field}")
    if idle["metrics"] is None:
        instrument_failures.append("idle:metric_missing")
    elif idle["metrics"]["percent_of_one_core"] > 0.25:
        resource_failures.append("idle:percent_of_one_core")
    if not idle["test_assertion_pass"]:
        instrument_failures.append("idle:test_assertion_failed")
    if idle["command"]["observed_affinity"] != [MEASUREMENT_CPU]:
        instrument_failures.append("idle:affinity_invalid")
    rss_by_label = {row["label"]: row for row in rss_rows}
    if any(row["rss_bytes"] is None or row["sample_count"] != 20 or row["error"] for row in rss_rows):
        instrument_failures.append("rss:incomplete")
        rss_delta = None
    else:
        rss_delta = max(0, rss_by_label["capture_on"]["rss_bytes"] - rss_by_label["capture_off"]["rss_bytes"])
        if rss_delta > 16 * 1024 * 1024:
            resource_failures.append("rss:delta")
    if not parity["byte_identical"] or parity["row_count"] != 16:
        resource_failures.append("parity:byte_identity")

    resource = {
        "schema": RESOURCE_SCHEMA,
        "candidate_commit": CANDIDATE_COMMIT,
        "measurement_cpu": MEASUREMENT_CPU,
        "round_count": ROUND_COUNT,
        "floor_probes": floors,
        "metrics": {
            "hot_latency": hot,
            "single_ledger_sync": single,
            "three_ledger_sync": three,
            "idle_cpu": idle,
            "rss": {"rows": rss_rows, "delta_bytes": rss_delta},
        },
        "resource_failures": sorted(resource_failures),
        "instrument_failures": sorted(instrument_failures),
        "all_pass_before_monitor": not resource_failures and not instrument_failures,
        "executable_set_root_sha256": executable_root,
        "oracle_ownership_root_sha256": ownership_root,
    }
    return resource, parity


def exact_untouched(before: dict[str, Any], current: dict[str, Any]) -> None:
    for unit in UNTOUCHED_UNITS:
        if current[unit] != before[unit]:
            raise GateFailure(f"untouched_service_changed:{unit}")


def semantic_health_equal(before: dict[str, Any], after: dict[str, Any]) -> bool:
    return all(before[label]["semantic"] == after[label]["semantic"] for label in before)


def verify_current_production() -> None:
    receipt = read_json(CURRENT_RECEIPT)
    if sha256_file(CURRENT_RECEIPT) != CURRENT_RECEIPT_FILE_SHA256:
        raise GateFailure("current_receipt_file_mismatch")
    if receipt.get("receipt_root_sha256") != CURRENT_RECEIPT_ROOT:
        raise GateFailure("current_receipt_embedded_root_mismatch")
    if receipt.get("source") != {"commit": CURRENT_RECEIPT_COMMIT, "tree": CURRENT_RECEIPT_TREE}:
        raise GateFailure("current_receipt_source_mismatch")
    checks = (
        (sha256_file(PRODUCTION_BINARY), BASELINE_BINARY_SHA256, "installed_binary"),
        (sha256_file(PRODUCTION_CONFIG), BASELINE_CONFIG_SHA256, "installed_config"),
        (sha256_file(UNIT_FILE), legacy.UNIT_SHA256, "unit"),
        (sha256_file(PHASE_CONFIG), legacy.PHASE_CONFIG_SHA256, "phase_config"),
        (sha256_file(AUTHORITY_CONFIG), legacy.AUTHORITY_CONFIG_SHA256, "authority_config"),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise GateFailure(f"STALE_BEFORE_MUTATION:{label}:{actual}")


def isolate_observer() -> None:
    available = set(os.sched_getaffinity(0))
    observer = available - {MEASUREMENT_CPU, MEASUREMENT_SIBLING}
    if not observer:
        raise GateFailure("observer_affinity_unavailable")
    os.sched_setaffinity(0, observer)


def prepare(args: argparse.Namespace) -> int:
    if os.geteuid() != 0:
        raise GateFailure("root_required")
    root = Path(args.transaction_directory)
    if root.exists():
        raise GateFailure(f"transaction_directory_exists:{root}")
    root.mkdir(parents=True, mode=0o700)
    (root / "evidence").mkdir(mode=0o700)
    try:
        verify_current_production()
        before_services = service_snapshot()
        require_active(before_services)
        before_health = health_snapshot()
        before_economics = economics_snapshot()
        if before_economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure(f"baseline_economics_unsafe:{before_economics}")
        before_probe = route_probe()
        before_journal = journal_snapshot()
        transition_rss_before = process_rss(before_services[TRANSITION_UNIT]["main_pid"])

        config_path = Path(args.candidate_config)
        oracle_source = Path(args.parity_source)
        oracle_lock = Path(args.oracle_lock)
        if sha256_file(config_path) != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("candidate_config_identity_drift")
        if sha256_file(oracle_source) != legacy.ORACLE_SOURCE_SHA256:
            raise GateFailure("oracle_source_identity_drift")
        if sha256_file(oracle_lock) != legacy.ORACLE_LOCK_SHA256:
            raise GateFailure("oracle_lock_identity_drift")
        connector_before = read_json(Path(args.connector_before))
        if connector_before.get("active_state") != "active":
            raise GateFailure("connector_before_inactive")

        work = Path(f"/home/e/.cache/nando-s1c3-{args.transaction_id}")
        if work.exists():
            raise GateFailure(f"work_directory_exists:{work}")
        work.mkdir(parents=True)
        shutil.chown(work, user="e", group="e")
        source = work / "candidate"
        run(["git", "clone", "--no-checkout", args.bundle, str(source)], as_user=True, timeout=300,
            log=root / "evidence" / "git-clone.log")
        run(["git", "checkout", "--detach", CANDIDATE_COMMIT], cwd=source, as_user=True, timeout=60,
            log=root / "evidence" / "candidate-checkout.log")
        if run(["git", "rev-parse", "HEAD"], cwd=source, as_user=True).stdout.decode().strip() != CANDIDATE_COMMIT:
            raise GateFailure("candidate_commit_mismatch")
        if run(["git", "rev-parse", "HEAD^{tree}"], cwd=source, as_user=True).stdout.decode().strip() != CANDIDATE_TREE:
            raise GateFailure("candidate_tree_mismatch")
        if sha256_file(source / "Cargo.lock") != CARGO_LOCK_SHA256:
            raise GateFailure("candidate_lock_mismatch")
        projection_hash = production_projection_sha256(source / PRODUCTION_PROJECTION_PATH)
        if projection_hash != PRODUCTION_PROJECTION_SHA256:
            raise GateFailure("candidate_production_projection_mismatch")
        baseline = work / "baseline"
        run(["git", "worktree", "add", "--detach", str(baseline), BASELINE_COMMIT], cwd=source, as_user=True,
            timeout=120, log=root / "evidence" / "baseline-checkout.log")
        if run(["git", "rev-parse", "HEAD^{tree}"], cwd=baseline, as_user=True).stdout.decode().strip() != BASELINE_TREE:
            raise GateFailure("baseline_tree_mismatch")

        target = work / "candidate-target"
        run(["/home/e/.cargo/bin/cargo", "build", "--release", "--locked", "-p", "nando-transition-serving", "--bin", "nando-transition-serving"],
            cwd=source, env={"CARGO_TARGET_DIR": str(target)}, as_user=True, timeout=1800,
            log=root / "evidence" / "candidate-build.log")
        candidate_binary = target / "release" / "nando-transition-serving"
        harnesses = {
            "response-actor": prebuild_test_harness(source, target, "nando-response-actor", root / "evidence" / "response-actor-test-build.log"),
            "transition-serving": prebuild_test_harness(source, target, "nando-transition-serving", root / "evidence" / "transition-serving-test-build.log"),
        }
        baseline_oracle, baseline_ownership = prebuild_oracle(oracle_source, oracle_lock, baseline, work, "baseline", root / "evidence")
        candidate_oracle, candidate_ownership = prebuild_oracle(oracle_source, oracle_lock, source, work, "candidate", root / "evidence")
        oracles = {"baseline": baseline_oracle, "candidate": candidate_oracle}
        ownership = build_ownership_receipt(args.transaction_id, {"baseline": baseline_ownership, "candidate": candidate_ownership})
        write_json(root / "oracle-ownership-receipt.json", ownership)
        wrapper, floor_script = write_measurement_scripts(work)
        filesystem = filesystem_identity(work)
        config = parse_env_file(config_path)
        isolate_observer()
        executable_identities = {
            "candidate-binary": executable_identity(candidate_binary, CANDIDATE_COMMIT),
            "test-response-actor": executable_identity(harnesses["response-actor"], CANDIDATE_COMMIT),
            "test-transition-serving": executable_identity(harnesses["transition-serving"], CANDIDATE_COMMIT),
            "parity-baseline": executable_identity(oracles["baseline"], BASELINE_COMMIT),
            "parity-candidate": executable_identity(oracles["candidate"], CANDIDATE_COMMIT),
            "python-runtime": executable_identity(Path("/usr/bin/python3"), "host-runtime"),
            "affinity-wrapper": executable_identity(wrapper, PAPER_COMMIT),
            "filesystem-floor-probe": executable_identity(floor_script, PAPER_COMMIT),
        }
        executable_root = sha256_bytes(canonical_bytes(executable_identities))
        if any(sha256_file(Path(row["path"])) != row["sha256"] for row in executable_identities.values()):
            raise GateFailure("measurement_executable_drift_before_start")

        measurement_started_at = utc_now()
        measurement_started_ns = time.monotonic_ns()
        monitor = MeasurementMonitor(args.transaction_id, executable_root)
        monitor.start()
        resource, parity = evaluate_measurement(
            candidate_binary, source, harnesses, config, work, root / "evidence", monitor,
            wrapper, floor_script, filesystem, executable_root, ownership["ownership_root_sha256"], oracles,
        )
        monitor_receipt = monitor.finish(measurement_started_at, measurement_started_ns)
        write_json(root / "measurement-monitor-receipt.json", monitor_receipt)
        executable_identities_after = {
            name: executable_identity(Path(row["path"]), row["source_identity"])
            for name, row in executable_identities.items()
        }
        if executable_identities_after != executable_identities:
            resource["instrument_failures"].append("executable_drift_after_measurement")
        if not monitor_receipt["instrument_pass"]:
            resource["instrument_failures"].append("monitor_instrument_failure")
        resource.update({
            "observed_at": utc_now(),
            "measurement_started_at": measurement_started_at,
            "measurement_started_monotonic_ns": measurement_started_ns,
            "measurement_finished_at": monitor_receipt["measurement_finished_at"],
            "measurement_finished_monotonic_ns": monitor_receipt["measurement_finished_monotonic_ns"],
            "monitor_root_sha256": monitor_receipt["monitor_root_sha256"],
            "parity_root_sha256": parity["parity_root_sha256"],
            "resource_failures": sorted(set(resource["resource_failures"])),
            "instrument_failures": sorted(set(resource["instrument_failures"])),
        })
        resource["all_pass"] = not resource["resource_failures"] and not resource["instrument_failures"]
        resource = add_root(resource, "resource_root_sha256")
        write_json(root / "resource-receipt.json", resource)
        write_json(root / "parity-receipt.json", parity)
        write_json(root / "executable-identities.json", add_root({
            "schema": "nando.s1c3b-executable-identities.v1",
            "transaction_id": args.transaction_id,
            "before": executable_identities,
            "after": executable_identities_after,
        }, "executable_identities_root_sha256"))

        if not resource["all_pass"]:
            write_json(root / "transaction-state.json", {
                "schema": STATE_SCHEMA,
                "state": "RESOURCE_VETO",
                "verdict": "S1C3B_RESOURCE_VETO",
                "transaction_id": args.transaction_id,
                "production_mutation": False,
                "resource_root_sha256": resource["resource_root_sha256"],
            }, 0o600)
            fsync_directory(root)
            print(json.dumps({"state": "RESOURCE_VETO", "resource_root_sha256": resource["resource_root_sha256"]}, sort_keys=True))
            return 3

        rollback_root = root / "rollback"
        rollback_root.mkdir(mode=0o700)
        rollback_files = {
            "nando-transition-serving": PRODUCTION_BINARY,
            "transition-serving.env": PRODUCTION_CONFIG,
            "nando-transition-serving.service": UNIT_FILE,
            "previous-deployment-receipt.json": CURRENT_RECEIPT,
        }
        for name, source_path in rollback_files.items():
            destination = rollback_root / name
            shutil.copy2(source_path, destination)
            os.chmod(destination, 0o500 if name == "nando-transition-serving" else 0o400)
        rollback_entries = [
            {"path": path.name, "sha256": sha256_file(path), "size_bytes": path.stat().st_size}
            for path in sorted(rollback_root.iterdir())
        ]
        rollback_manifest = "".join(
            f'{row["sha256"]} {row["size_bytes"]} {row["path"]}\n' for row in rollback_entries
        ).encode()
        atomic_write(root / "rollback-manifest.sha256", rollback_manifest, 0o400)
        shutil.copy2(candidate_binary, root / "candidate-binary")
        shutil.copy2(config_path, root / "candidate-config")
        os.chmod(root / "candidate-binary", 0o500)
        os.chmod(root / "candidate-config", 0o400)
        candidate_hash = sha256_file(candidate_binary)
        preparation = add_root({
            "schema": PREPARATION_SCHEMA,
            "transaction_id": args.transaction_id,
            "state": "PREPARED",
            "created_at": utc_now(),
            "paper": {
                "commit": PAPER_COMMIT,
                "tree": PAPER_TREE,
                "manifest_root_sha256": PAPER_MANIFEST_ROOT,
                "verification_sha256": PAPER_VERIFICATION_SHA256,
                "critique_sha256": PAPER_CRITIQUE_SHA256,
            },
            "candidate": {
                "source_commit": CANDIDATE_COMMIT,
                "source_tree": CANDIDATE_TREE,
                "cargo_lock_sha256": CARGO_LOCK_SHA256,
                "binary_sha256": candidate_hash,
                "binary_size_bytes": candidate_binary.stat().st_size,
                "config_sha256": CANDIDATE_CONFIG_SHA256,
                "production_projection_path": str(PRODUCTION_PROJECTION_PATH),
                "production_projection_schema": PRODUCTION_PROJECTION_SCHEMA,
                "production_projection_sha256": projection_hash,
            },
            "production": {
                "receipt_path": str(CURRENT_RECEIPT),
                "receipt_root_sha256": CURRENT_RECEIPT_ROOT,
                "source_commit": CURRENT_RECEIPT_COMMIT,
                "source_tree": CURRENT_RECEIPT_TREE,
                "binary_sha256": BASELINE_BINARY_SHA256,
                "config_sha256": BASELINE_CONFIG_SHA256,
            },
            "services_before": before_services,
            "health_before": before_health,
            "economics_before": before_economics,
            "route_probe_before": before_probe,
            "connector_before": connector_before,
            "journal_before": before_journal,
            "transition_rss_before": transition_rss_before,
            "measurement_cpu": MEASUREMENT_CPU,
            "executable_set_root_sha256": executable_root,
            "oracle_ownership_root_sha256": ownership["ownership_root_sha256"],
            "monitor_root_sha256": monitor_receipt["monitor_root_sha256"],
            "resource_root_sha256": resource["resource_root_sha256"],
            "parity_root_sha256": parity["parity_root_sha256"],
            "rollback": {"manifest_root_sha256": sha256_bytes(rollback_manifest), "entries": rollback_entries},
        }, "preparation_root_sha256")
        write_json(root / "preparation.json", preparation)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "PREPARED", "transaction_id": args.transaction_id}, 0o600)
        fsync_directory(root)
        print(json.dumps({"state": "PREPARED", "preparation_root_sha256": preparation["preparation_root_sha256"]}, sort_keys=True))
        return 0
    except Exception as error:
        write_json(root / "preflight-failure.json", {
            "schema": "nando.s1c3b-preflight-failure.v1",
            "observed_at": utc_now(),
            "error": str(error),
        })
        write_json(root / "transaction-state.json", {
            "schema": STATE_SCHEMA,
            "state": "PREFLIGHT_FAILURE",
            "transaction_id": args.transaction_id,
            "production_mutation": False,
        }, 0o600)
        fsync_directory(root)
        raise


def cleanup_empty_new_journal(before: dict[str, Any]) -> None:
    if before.get("present") or not JOURNAL.exists():
        return
    after = journal_snapshot()
    if after.get("total_bytes") == 0 and not after.get("entries"):
        JOURNAL.rmdir()


def rollback(root: Path, reason: str) -> None:
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {
        "ROLLBACK_ARMED",
        "FINALIZE_PENDING",
        "FINAL_VERIFICATION_PENDING",
    }:
        raise GateFailure(f"rollback_state_invalid:{state}")
    preparation = read_json(root / "preparation.json")
    forward_journal = journal_snapshot()
    systemctl("stop", TRANSITION_UNIT, check=False)
    install_pair(root / "rollback" / "nando-transition-serving", root / "rollback" / "transition-serving.env")
    if sha256_file(PRODUCTION_BINARY) != BASELINE_BINARY_SHA256 or sha256_file(PRODUCTION_CONFIG) != BASELINE_CONFIG_SHA256:
        raise GateFailure("rollback_pair_restore_failed")
    systemctl("start", TRANSITION_UNIT)
    services_after, health_after = wait_for_service()
    time.sleep(15)
    services_survival, health_survival = wait_for_service()
    exact_untouched(preparation["services_before"], services_after)
    exact_untouched(preparation["services_before"], services_survival)
    cleanup_empty_new_journal(preparation["journal_before"])
    pending = {
        "schema": PENDING_SCHEMA,
        "verdict": "S1C3B_ROLLBACK_PASS",
        "rollback_reason": reason,
        "services_after": services_after,
        "services_survival": services_survival,
        "health_after": health_after,
        "health_survival": health_survival,
        "route_probe_after": route_probe(),
        "route_probe_survival": route_probe(),
        "journal_after": journal_snapshot(prefix_reference=forward_journal),
        "capture_environment": {},
        "capture_available": False,
        "startup_log_clean": True,
        "health_semantics_preserved": semantic_health_equal(preparation["health_before"], health_after)
        and semantic_health_equal(preparation["health_before"], health_survival),
        "route_probe_equivalent": route_probe() == preparation["route_probe_before"],
        "active_packages_preserved": True,
        "economics": economics_snapshot(),
        "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
        "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
        "transition_rss_after": process_rss(services_survival[TRANSITION_UNIT]["main_pid"]),
        "immutable_after": {
            "unit_sha256": sha256_file(UNIT_FILE),
            "phase_config_sha256": sha256_file(PHASE_CONFIG),
            "authority_config_sha256": sha256_file(AUTHORITY_CONFIG),
        },
    }
    write_json(root / "pending-receipt.json", pending, 0o600)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_PENDING", "transaction_id": preparation["transaction_id"]}, 0o600)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    verification = read_json(Path(args.predeployment_verification))
    verification_root = verification.get("predeployment_verification_root_sha256")
    if sha256_bytes(canonical_bytes(verification, "predeployment_verification_root_sha256")) != verification_root:
        raise GateFailure("predeployment_verification_root_mismatch")
    expected = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3B_PREPARATION_PASS",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": preparation["oracle_ownership_root_sha256"],
        "monitor_root_sha256": preparation["monitor_root_sha256"],
        "resource_root_sha256": preparation["resource_root_sha256"],
        "parity_root_sha256": preparation["parity_root_sha256"],
    }
    if {key: verification.get(key) for key in expected} != expected:
        raise GateFailure("predeployment_verification_receipt_mismatch")
    state = read_json(root / "transaction-state.json")
    if state.get("state") != "PREPARED":
        raise GateFailure(f'transaction_not_prepared:{state.get("state")}')
    write_json(root / "predeployment-verification.json", verification)
    verify_current_production()
    current_services = service_snapshot()
    if current_services != preparation["services_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:services")
    if not semantic_health_equal(preparation["health_before"], health_snapshot()):
        raise GateFailure("STALE_BEFORE_MUTATION:health")
    if economics_snapshot() != preparation["economics_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:economics")
    if route_probe() != preparation["route_probe_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:route")
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_ARMED", "transaction_id": preparation["transaction_id"]}, 0o600)
    old_pid = preparation["services_before"][TRANSITION_UNIT]["main_pid"]
    stopped = False
    try:
        systemctl("stop", TRANSITION_UNIT)
        stopped = True
        deadline = time.monotonic() + 20
        while Path(f"/proc/{old_pid}").exists() and time.monotonic() < deadline:
            time.sleep(0.1)
        if Path(f"/proc/{old_pid}").exists():
            raise GateFailure("old_transition_pid_did_not_exit")
        install_pair(root / "candidate-binary", root / "candidate-config")
        if sha256_file(PRODUCTION_BINARY) != preparation["candidate"]["binary_sha256"]:
            raise GateFailure("candidate_binary_install_mismatch")
        if sha256_file(PRODUCTION_CONFIG) != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("candidate_config_install_mismatch")
        systemctl("start", TRANSITION_UNIT)
        services_after, health_after = wait_for_service()
        exact_untouched(preparation["services_before"], services_after)
        new_pid = services_after[TRANSITION_UNIT]["main_pid"]
        if new_pid == old_pid:
            raise GateFailure("transition_pid_did_not_change")
        if services_after[TRANSITION_UNIT]["nrestarts"] != current_services[TRANSITION_UNIT]["nrestarts"]:
            raise GateFailure("transition_nrestarts_changed")
        environment = process_environment(new_pid)
        expected_environment = {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": str(JOURNAL),
        }
        if {key: environment.get(key) for key in expected_environment} != expected_environment:
            raise GateFailure("capture_environment_mismatch")
        if not JOURNAL.exists():
            raise GateFailure("capture_journal_not_open")
        transition_rss_after = process_rss(new_pid)
        if max(0, transition_rss_after - preparation["transition_rss_before"]) > 16 * 1024 * 1024:
            raise GateFailure("production_rss_delta_exceeded")
        started_at = time.time()
        logs = run(
            ["journalctl", "-u", TRANSITION_UNIT, "--since", f"@{started_at - 5:.6f}", "--no-pager", "-o", "cat"],
            timeout=10,
        ).stdout
        legacy.write_evidence(root, "startup.log", logs)
        startup_log_clean = b"nando-grounded-decision shadow unavailable" not in logs
        if not startup_log_clean:
            raise GateFailure("grounded_decision_startup_unavailable")
        after_journal = journal_snapshot(prefix_reference=preparation["journal_before"])
        if after_journal["raw_payload_bytes"] != 0 or after_journal["total_bytes"] > 2 * 1024 * 1024 * 1024:
            raise GateFailure("journal_safety_failed")
        after_probe = route_probe()
        if after_probe != preparation["route_probe_before"]:
            raise GateFailure("post_route_probe_mismatch")
        if not semantic_health_equal(preparation["health_before"], health_after):
            raise GateFailure("post_health_semantics_changed")
        active_packages_preserved = (
            health_after["hot"]["semantic"].get("response_active_profiles")
            == preparation["health_before"]["hot"]["semantic"].get("response_active_profiles")
        )
        if not active_packages_preserved:
            raise GateFailure("active_packages_changed")
        time.sleep(15)
        services_survival, health_survival = wait_for_service()
        exact_untouched(preparation["services_before"], services_survival)
        if services_survival[TRANSITION_UNIT]["main_pid"] != new_pid:
            raise GateFailure("transition_pid_changed_during_survival")
        if services_survival[TRANSITION_UNIT]["nrestarts"] != current_services[TRANSITION_UNIT]["nrestarts"]:
            raise GateFailure("transition_restart_during_survival")
        economics = economics_snapshot()
        if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure("post_install_economics_unsafe")
        route_probe_survival = route_probe()
        if route_probe_survival != preparation["route_probe_before"]:
            raise GateFailure("survival_route_probe_mismatch")
        pending = {
            "schema": PENDING_SCHEMA,
            "verdict": "S1C3B_DEPLOYMENT_PASS",
            "services_after": services_after,
            "services_survival": services_survival,
            "health_after": health_after,
            "health_survival": health_survival,
            "route_probe_after": after_probe,
            "route_probe_survival": route_probe_survival,
            "journal_after": journal_snapshot(prefix_reference=preparation["journal_before"]),
            "capture_environment": expected_environment,
            "capture_available": True,
            "startup_log_clean": startup_log_clean,
            "health_semantics_preserved": semantic_health_equal(preparation["health_before"], health_after)
            and semantic_health_equal(preparation["health_before"], health_survival),
            "route_probe_equivalent": route_probe() == preparation["route_probe_before"],
            "active_packages_preserved": active_packages_preserved,
            "economics": economics,
            "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
            "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
            "transition_rss_after": transition_rss_after,
            "immutable_after": {
                "unit_sha256": sha256_file(UNIT_FILE),
                "phase_config_sha256": sha256_file(PHASE_CONFIG),
                "authority_config_sha256": sha256_file(AUTHORITY_CONFIG),
            },
        }
        write_json(root / "pending-receipt.json", pending, 0o600)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "FINALIZE_PENDING", "transaction_id": preparation["transaction_id"]}, 0o600)
        return 0
    except Exception as error:
        if stopped:
            try:
                rollback(root, str(error))
            except Exception as rollback_error:
                raise GateFailure(f"S1C3B_VETO:{error}:rollback:{rollback_error}") from rollback_error
            raise GateFailure(f"S1C3B_ROLLBACK_PASS:{error}") from error
        raise


def connector_failure_reasons(
    before: dict[str, Any], after: dict[str, Any]
) -> list[str]:
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


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    pending = read_json(root / "pending-receipt.json")
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {"FINALIZE_PENDING", "ROLLBACK_PENDING"}:
        raise GateFailure(f"finalize_state_invalid:{state}")
    connector_after = read_json(Path(args.connector_after))
    connector_before = preparation["connector_before"]
    veto_reasons = connector_failure_reasons(connector_before, connector_after)
    if veto_reasons and state == "FINALIZE_PENDING":
        rollback(root, ",".join(veto_reasons))
        pending = read_json(root / "pending-receipt.json")
    if veto_reasons:
        pending["verdict"] = "S1C3B_VETO"
        pending["veto_reasons"] = veto_reasons
    economics = pending["economics"]
    receipt = add_root({
        "schema": RECEIPT_SCHEMA,
        "transaction_id": preparation["transaction_id"],
        "verdict": pending["verdict"],
        "finalized_at": utc_now(),
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": preparation["oracle_ownership_root_sha256"],
        "monitor_root_sha256": preparation["monitor_root_sha256"],
        "executable_set_root_sha256": preparation["executable_set_root_sha256"],
        "resource_root_sha256": preparation["resource_root_sha256"],
        "parity_root_sha256": preparation["parity_root_sha256"],
        "predeployment_verification_root_sha256": read_json(root / "predeployment-verification.json")["predeployment_verification_root_sha256"],
        "services_before": preparation["services_before"],
        "services_after": pending["services_after"],
        "services_survival": pending["services_survival"],
        "health_before": preparation["health_before"],
        "health_after": pending["health_after"],
        "health_survival": pending["health_survival"],
        "route_probe_before": preparation["route_probe_before"],
        "route_probe_after": pending["route_probe_after"],
        "route_probe_survival": pending["route_probe_survival"],
        "connector_before": connector_before,
        "connector_after": connector_after,
        "installed_binary_sha256": pending["installed_binary_sha256"],
        "installed_config_sha256": pending["installed_config_sha256"],
        "immutable_after": pending["immutable_after"],
        "capture_environment": pending["capture_environment"],
        "capture_available": pending["capture_available"],
        "startup_log_clean": pending["startup_log_clean"],
        "health_semantics_preserved": pending["health_semantics_preserved"],
        "route_probe_equivalent": pending["route_probe_equivalent"],
        "active_packages_preserved": pending["active_packages_preserved"],
        "false_accepts_after": economics["false_accepts"],
        "runtime_parity_failures_after": economics["runtime_parity_mismatches"],
        "journal_before": preparation["journal_before"],
        "journal_after": pending["journal_after"],
        "transition_rss_before": preparation["transition_rss_before"],
        "transition_rss_after": pending["transition_rss_after"],
        "survival_seconds": 15,
        "veto_reasons": pending.get("veto_reasons", []),
    }, "receipt_root_sha256")
    write_json(root / "deployment-receipt.json", receipt)
    write_json(root / "transaction-state.json", {
        "schema": STATE_SCHEMA,
        "state": "FINAL_VERIFICATION_PENDING",
        "transaction_id": preparation["transaction_id"],
        "verdict": receipt["verdict"],
    }, 0o600)
    return 0


def seal(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json")
    if state.get("state") != "FINAL_VERIFICATION_PENDING":
        raise GateFailure(f'final_seal_state_invalid:{state.get("state")}')
    receipt = read_json(root / "deployment-receipt.json")
    verification = read_json(Path(args.final_verification))
    verification_root = verification.get("final_verification_root_sha256")
    if sha256_bytes(
        canonical_bytes(verification, "final_verification_root_sha256")
    ) != verification_root:
        raise GateFailure("final_verification_root_mismatch")
    expected = {
        "schema": FINAL_VERIFICATION_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": receipt["verdict"],
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": receipt["preparation_root_sha256"],
    }
    if {key: verification.get(key) for key in expected} != expected:
        raise GateFailure("final_verification_receipt_mismatch")
    write_json(root / "final-verification.json", verification, 0o400)
    write_json(root / "transaction-state.json", {
        "schema": STATE_SCHEMA,
        "state": "COMPLETE",
        "transaction_id": receipt["transaction_id"],
        "verdict": receipt["verdict"],
        "final_verification_root_sha256": verification_root,
    }, 0o600)
    return 0


def rollback_command(args: argparse.Namespace) -> int:
    rollback(Path(args.transaction_directory), args.reason)
    return 0


def locked_command(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    lock_path = root / ".mutation.lock"
    descriptor = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        if args.command == "execute":
            return execute(args)
        if args.command == "finalize":
            return finalize(args)
        if args.command == "seal":
            return seal(args)
        return rollback_command(args)
    finally:
        os.close(descriptor)


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--transaction-id", required=True)
    prepare_parser.add_argument("--transaction-directory", required=True)
    prepare_parser.add_argument("--bundle", required=True)
    prepare_parser.add_argument("--candidate-config", required=True)
    prepare_parser.add_argument("--parity-source", required=True)
    prepare_parser.add_argument("--oracle-lock", required=True)
    prepare_parser.add_argument("--connector-before", required=True)
    execute_parser = subparsers.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    execute_parser.add_argument("--predeployment-verification", required=True)
    finalize_parser = subparsers.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    seal_parser = subparsers.add_parser("seal")
    seal_parser.add_argument("--transaction-directory", required=True)
    seal_parser.add_argument("--final-verification", required=True)
    rollback_parser = subparsers.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            return prepare(args)
        return locked_command(args)
    except GateFailure as error:
        print(json.dumps({"schema": "nando.s1c3b-executor-error.v1", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 3


if __name__ == "__main__":
    raise SystemExit(main())
